import oci


config = oci.config.from_file()
print(config)
core_client = oci.core.ComputeClient(config)
instance_action_response = core_client.instance_action(
    instance_id="ocid1.instance.oc1.us-chicago-1.anxxeljreamwweaczolj43ykq53jdlavbyliuvfav2x3vs4dcp7prih5wlva",
    action="RESET",
    instance_power_action_details=oci.core.models.ResetActionDetails(
        action_type="reset"
    ),
)
print(instance_action_response.data)
